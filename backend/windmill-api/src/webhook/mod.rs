use std::{collections::HashMap, hash::Hash};

use axum::response::{IntoResponse, Response};
use base64::{
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE},
    Engine,
};
use hmac::{Hmac, Mac};
use http::{HeaderMap, HeaderValue, StatusCode};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
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
use github::{Github, GITHUB_WEBHOOK_VALIDATOR};
use shopify::{Shopify, SHOPIFY_WEBHOOK_VALIDATOR};
use slack::{Slack, SLACK_WEBHOOK_VALIDATOR};
use stripe::{Stripe, STRIPE_WEBHOOK_VALIDATOR};
use tiktok::{TikTok, TIKTOK_WEBHOOK_VALIDATOR};
use twitch::{Twitch, TWITCH_WEBHOOK_VALIDATOR};
use zoom::{Zoom, ZOOM_WEBHOOK_VALIDATOR};

fn deserialize_limited_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<String> = Vec::deserialize(deserializer)?;

    if vec.len() > 2 {
        return Err(serde::de::Error::custom(
            "minimum 1 separator, max 2 separators",
        ));
    }

    Ok(vec)
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

pub trait WebhookVerifier {
    fn validate_hmac_signature(
        &self,
        headers: &HeaderMap,
        secret: &str,
        raw_payload: &str,
    ) -> Result<WebhookRequestType, WebhookError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HmacPayload<'headers> {
    pub signature_to_verify: &'headers str,
    pub payload: String,
}

impl<'headers> HmacPayload<'headers> {
    pub fn new(signature_to_verify: &'headers str, payload: String) -> Self {
        Self { signature_to_verify, payload }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsingRules {
    #[serde(deserialize_with = "deserialize_limited_vec")]
    pub separators: Vec<String>,
    pub signature_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureParse {
    pub signature_header_name: String,
    pub parsing_rules: Option<ParsingRules>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SignatureLocation {
    Header(SignatureParse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadConstruction {
    pub signature_location: SignatureLocation,
    pub payload_format: Vec<String>,
    pub payload_separator: Option<String>,
    pub include_raw_body_at_end_of_payload: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookHmacValidator {
    pub prefix: Option<String>,
    pub payload_construction: PayloadConstruction,
    pub signature_encoding: Encoding,
    pub algorithm: HmacAlgorithm,
}

impl WebhookHmacValidator {
    fn construct_payload<'headers>(
        &self,
        headers: &'headers HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacPayload<'headers>, WebhookError> {
        let mut signature_headers_key: HashMap<&str, &str> = HashMap::new();

        let signature_to_verify = match &self.payload_construction.signature_location {
            SignatureLocation::Header(signature_parser) => {
                match signature_parser.parsing_rules.as_ref() {
                    //Check if user has defined separator for headers e.g [",", "="] and header value is t=2154156654,s=dieri
                    //first split by "," => ["t=2154156654", "s=dieri"]
                    //second split "=" store in hashmap with key t=>897609600 and s=dieri
                    Some(parsing_rules) => {
                        let signature = headers
                            .try_get_webhook_header(&signature_parser.signature_header_name)?;

                        let separators = &parsing_rules.separators;

                        signature_headers_key = signature
                            .split(separators.get(0).unwrap())
                            .map(|header| {
                                let mut key_and_value = header.split(separators.get(1).unwrap());
                                let key = key_and_value.next();
                                let value = key_and_value.next();
                                (key, value)
                            })
                            .filter_map(|(key, value)| match (key, value) {
                                (Some(key), Some(value)) => Some((key, value)),
                                _ => None,
                            })
                            .collect();

                        let key = &parsing_rules.signature_key;

                        //based on the example above the key here is `s` and it retrieves the signature value: dieri
                        let signature = *signature_headers_key.get(key.as_str()).ok_or(
                            WebhookError::InvalidConfig(
                                "invalid key for signature retrieval".to_string(),
                            ),
                        )?;

                        signature
                    }
                    None => {
                        headers.try_get_webhook_header(&signature_parser.signature_header_name)?
                    }
                }
            }
        };

        let payload_format = &self.payload_construction.payload_format;

        let include_body = self.payload_construction.include_raw_body_at_end_of_payload;
        let mut payload = Vec::with_capacity(payload_format.len() + 1);

        for format in payload_format {
            if format.starts_with("#") && format.len() > 1 {
                let value =
                    *signature_headers_key
                        .get(&format[1..])
                        .ok_or(WebhookError::InvalidHeader(format!(
                            "Missing key {} in headers",
                            &format[1..]
                        )))?;
                payload.push(value);
            } else if format.starts_with("!") && format.len() > 1 {
                payload.push(&format[1..])
            } else {
                payload.push(headers.try_get_webhook_header(&format)?);
            }
        }

        if include_body {
            payload.push(raw_payload);
        }

        let payload = if let Some(payload_separator) = &self.payload_construction.payload_separator
        {
            payload.join(&payload_separator)
        } else {
            payload.join("")
        };

        Ok(HmacPayload::new(signature_to_verify, payload))
    }
}

impl WebhookVerifier for WebhookHmacValidator {
    fn validate_hmac_signature(
        &self,
        headers: &HeaderMap,
        secret: &str,
        raw_payload: &str,
    ) -> Result<WebhookRequestType, WebhookError> {
        let hmac = self.construct_payload(headers, raw_payload)?;

        let hmac_signature = calculate_hmac_signature(self.algorithm, &secret, &hmac.payload);

        let encoded_signature = encode_hmac_signature(self.signature_encoding, &hmac_signature);

        let final_expected_signature = if let Some(signature_prefix) = self.prefix.as_ref() {
            format!("{}{}", signature_prefix, encoded_signature)
        } else {
            encoded_signature
        };

        if !constant_time_eq(
            final_expected_signature.as_bytes(),
            hmac.signature_to_verify.as_bytes(),
        ) {
            return Err(WebhookError::InvalidSignature);
        }

        Ok(WebhookRequestType::Event)
    }
}

pub trait WebhookHandler {
    fn handle_challenge_request<'header>(
        &self,
        headers: &'header HeaderMap,
        authentication_method: &WebhookAuthenticationMethod,
        raw_payload: &str,
    ) -> Result<Option<Response>, WebhookError>;
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
pub struct HmacAuthentication {
    pub webhook_signing_secret: String,
    #[serde(flatten)]
    pub config: Option<WebhookHmacValidator>,
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
#[serde(untagged)]
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
    pub fn get_webhook_handler(
        &self,
    ) -> Option<(&'static dyn WebhookHandler, &WebhookHmacValidator)> {
        let handler: (&'static dyn WebhookHandler, &WebhookHmacValidator) = match *self {
            WebhookType::Github => (&Github, &GITHUB_WEBHOOK_VALIDATOR),
            WebhookType::Shopify => (&Shopify, &SHOPIFY_WEBHOOK_VALIDATOR),
            WebhookType::Slack => (&Slack, &SLACK_WEBHOOK_VALIDATOR),
            WebhookType::Stripe => (&Stripe, &STRIPE_WEBHOOK_VALIDATOR),
            WebhookType::TikTok => (&TikTok, &TIKTOK_WEBHOOK_VALIDATOR),
            WebhookType::Twitch => (&Twitch, &TWITCH_WEBHOOK_VALIDATOR),
            WebhookType::Zoom => (&Zoom, &ZOOM_WEBHOOK_VALIDATOR),
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

pub fn encode_hmac_signature(encoding: Encoding, hmac_signature: &[u8]) -> String {
    match encoding {
        Encoding::Hex => hex::encode(hmac_signature),
        Encoding::Base64 => BASE64_STANDARD.encode(hmac_signature),
        Encoding::Base64Uri => BASE64_URL_SAFE.encode(hmac_signature),
    }
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
            .map(|(handler, _)| {
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
            }) => match handler {
                Some((_, validator)) => {
                    validator.validate_hmac_signature(
                        headers,
                        webhook_signing_secret,
                        raw_payload,
                    )?;
                }
                None => {
                    let config = config
                        .as_ref()
                        .ok_or(WebhookError::InvalidConfig("Missing config".to_string()))?;

                    config.validate_hmac_signature(headers, webhook_signing_secret, raw_payload)?;
                }
            },
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

    #[error("invalid webhook config: {0}")]
    InvalidConfig(String),
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

            WebhookError::InvalidConfig(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = json!({ "error": error_message });

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        (status, headers, body.to_string()).into_response()
    }
}
