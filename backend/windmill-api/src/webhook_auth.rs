use std::{collections::HashMap, hash::Hash};

use axum::response::{IntoResponse, Response};
use base64::{
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE},
    Engine,
};
use github::GITHUB_WEBHOOK_VALIDATOR;
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

use constant_time_eq::constant_time_eq;
use shopify::SHOPIFY_WEBHOOK_VALIDATOR;
use slack::SLACK_WEBHOOK_VALIDATOR;
use stripe::STRIPE_WEBHOOK_VALIDATOR;
use tiktok::TIKTOK_WEBHOOK_VALIDATOR;
use twitch::{Twitch, TWITCH_WEBHOOK_VALIDATOR};
use zoom::{Zoom, ZOOM_WEBHOOK_VALIDATOR};

mod github {
    use super::*;

    lazy_static::lazy_static! {
        pub static ref GITHUB_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: Some("sha256=".to_string()),
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "X-Hub-Signature-256".to_string(),
                    parsing_rules: None,
                }),
                payload_format: vec![],
                payload_separator: None,
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Hex,
            algorithm: HmacAlgorithm::Sha256,
        };
    }
}

mod shopify {
    use super::*;

    lazy_static::lazy_static! {
        pub static ref SHOPIFY_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: None,
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "X-Shopify-Hmac-Sha256".to_string(),
                    parsing_rules: None,
                }),
                payload_format: vec![],
                payload_separator: None,
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Base64,
            algorithm: HmacAlgorithm::Sha256,
        };
    }
}

mod slack {
    use super::*;

    lazy_static::lazy_static! {
        pub static ref SLACK_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: Some("v0=".to_string()),
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "X-Slack-Signature".to_string(),
                    parsing_rules: None,
                }),
                payload_format: vec!["raw#v0".to_string(), "h#X-Slack-Request-Timestamp".to_string()],
                payload_separator: Some(":".to_string()),
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Hex,
            algorithm: HmacAlgorithm::Sha256,
        };
    }
}

mod stripe {
    use super::*;

    lazy_static::lazy_static! {
        pub static ref STRIPE_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: None,
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "STRIPE-SIGNATURE".to_string(),
                    parsing_rules: Some(ParsingRules {
                        separators: vec![",".to_string(), "=".to_string()],
                        signature_key: "v1".to_string(),
                    }),
                }),
                payload_format: vec!["sh#t".to_string()],
                payload_separator: Some(".".to_string()),
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Hex,
            algorithm: HmacAlgorithm::Sha256,
        };
    }
}

mod tiktok {
    use super::*;

    lazy_static::lazy_static! {
        pub static ref TIKTOK_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: None,
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "TikTok-Signature".to_string(),
                    parsing_rules: Some(ParsingRules {
                        separators: vec![",".to_string(), "=".to_string()],
                        signature_key: "s".to_string(),
                    }),
                }),
                payload_format: vec!["sh#t".to_string()],
                payload_separator: Some(".".to_string()),
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Hex,
            algorithm: HmacAlgorithm::Sha256,
        };
    }
}

mod twitch {
    use super::*;
    use axum::body::Body;
    use serde_json::value::RawValue;

    lazy_static::lazy_static! {
        pub static ref TWITCH_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: Some("sha256=".to_string()),
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "Twitch-Eventsub-Message-Signature".to_string(),
                    parsing_rules: None,
                }),
                payload_format: vec!["h#Twitch-Eventsub-Message-Id".to_string(), "h#Twitch-Eventsub-Message-Timestamp".to_string()],
                payload_separator: None,
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Hex,
            algorithm: HmacAlgorithm::Sha256,
        };
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct TwitchCrcBody {
        challenge: String,
        subscription: Box<RawValue>,
        transport: Box<RawValue>,
        created_at: String,
    }

    pub struct Twitch;

    impl WebhookHandler for Twitch {
        fn handle_challenge_request<'header>(
            &self,
            headers: &'header HeaderMap,
            authentication_data: &WebhookProviders,
            raw_payload: &str,
        ) -> Result<Option<Response>, WebhookError> {
            let Some(secret) = authentication_data.webhook_signing_secret.as_ref() else {
                return Err(WebhookError::InvalidConfig(
                    "Twitch: Missing secret key for challenge request".to_string(),
                ));
            };

            TWITCH_WEBHOOK_VALIDATOR.validate_hmac_signature(headers, secret, raw_payload)?;

            let twitch_eventsub_message_type =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Type")?;

            if twitch_eventsub_message_type != "webhook_callback_verification" {
                return Ok(None);
            }

            let twitch_crc_body =
                serde_json::from_str::<TwitchCrcBody>(raw_payload).map_err(|e| {
                    WebhookError::InvalidChallengeResponse(format!("Twitch :{}", e.to_string()))
                })?;

            let response = Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body(Body::from(twitch_crc_body.challenge))
                .unwrap();

            Ok(Some(response))
        }
    }
}

mod zoom {
    use super::*;
    use axum::body::Body;
    use http::header;

    lazy_static::lazy_static! {
        pub static ref ZOOM_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
            prefix: Some("v0=".to_string()),
            payload_construction: PayloadConstruction {
                signature_location: SignatureLocation::Header(SignatureParse {
                    signature_header_name: "x-zm-signature".to_string(),
                    parsing_rules: None,
                }),
                payload_format: vec!["raw#v0".to_string(), "h#x-zm-request-timestamp".to_string()],
                payload_separator: Some(":".to_string()),
                body: BodyParsing::default()
            },
            signature_encoding: Encoding::Hex,
            algorithm: HmacAlgorithm::Sha256,
        };
    }

    pub struct Zoom;

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

    impl WebhookHandler for Zoom {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            authentication_data: &WebhookProviders,
            raw_payload: &str,
        ) -> Result<Option<Response>, WebhookError> {
            let Ok(zoom_request_body) = serde_json::from_str::<ZoomChallengeResponse>(raw_payload)
            else {
                return Ok(None);
            };

            println!("Zoom request body: {:?}", &zoom_request_body);

            if zoom_request_body.event != "endpoint.url_validation" {
                return Ok(None);
            }

            let Some(secret) = authentication_data.webhook_signing_secret.as_ref() else {
                return Err(WebhookError::InvalidConfig(
                    "Missing secret key for zoom challenge request".to_string(),
                ));
            };

            let hmac_signature = calculate_hmac_signature(
                HmacAlgorithm::Sha256,
                secret,
                &zoom_request_body.payload.plain_token,
            );

            let encoded_hmac_signature = encode_hmac_signature(Encoding::Hex, &hmac_signature);

            let response_body = json!({
                "plainToken": zoom_request_body.payload.plain_token,
                "encryptedToken": encoded_hmac_signature
            })
            .to_string();

            let response = Response::builder()
                .status(200)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(response_body))
                .unwrap();
            return Ok(Some(response));
        }
    }
}

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
pub struct BodyParsing {
    pub include_body: bool,
}

#[allow(unused)]
impl BodyParsing {
    pub fn new(include_body: bool) -> Self {
        Self { include_body }
    }
}

impl Default for BodyParsing {
    fn default() -> Self {
        Self { include_body: true }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadConstruction {
    pub signature_location: SignatureLocation,
    pub payload_format: Vec<String>,
    pub payload_separator: Option<String>,
    pub body: BodyParsing,
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

        let include_body = self.payload_construction.body.include_body;
        let mut payload = Vec::with_capacity(payload_format.len() + 1);

        for format in payload_format {
            if format.starts_with("sh#") && format.len() > 3 {
                let value =
                    *signature_headers_key
                        .get(&format[3..])
                        .ok_or(WebhookError::InvalidHeader(format!(
                            "Missing key {} in headers",
                            &format[1..]
                        )))?;
                payload.push(value);
            } else if format.starts_with("raw#") && format.len() > 4 {
                payload.push(&format[4..])
            } else if format.starts_with("h#") && format.len() > 2 {
                payload.push(headers.try_get_webhook_header(&format[2..])?);
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
        authentication_data: &WebhookProviders,
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
    pub config: WebhookHmacValidator,
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
pub struct WebhookProviders {
    pub webhook_type: WebhookType,
    webhook_signing_secret: Option<String>,
    webhook_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebhookAuthenticationMethod {
    ApiKey(ApiKeyAuthentication),
    BasicAuth(BasicAuthAuthentication),
    PredefinedProviders(WebhookProviders),
    HMAC(HmacAuthentication),
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
}

impl WebhookType {
    pub fn get_webhook_handler(
        &self,
    ) -> (Option<&'static dyn WebhookHandler>, &WebhookHmacValidator) {
        let handler: (Option<&'static dyn WebhookHandler>, &WebhookHmacValidator) = match *self {
            WebhookType::Github => (None, &GITHUB_WEBHOOK_VALIDATOR),
            WebhookType::Shopify => (None, &SHOPIFY_WEBHOOK_VALIDATOR),
            WebhookType::Slack => (None, &SLACK_WEBHOOK_VALIDATOR),
            WebhookType::Stripe => (None, &STRIPE_WEBHOOK_VALIDATOR),
            WebhookType::TikTok => (None, &TIKTOK_WEBHOOK_VALIDATOR),
            WebhookType::Twitch => (Some(&Twitch), &TWITCH_WEBHOOK_VALIDATOR),
            WebhookType::Zoom => (Some(&Zoom), &ZOOM_WEBHOOK_VALIDATOR),
            //WebhookType::Paypal => (&Zoom, &PAYPAL_WEBHOOK_VALIDATOR),
        };
        handler
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
    pub authentication_method: WebhookAuthenticationMethod,
}

impl Webhook {
    pub fn verify_signatures(
        &self,
        headers: &HeaderMap,
        raw_payload: &str,
    ) -> Result<WebhookRequestType, WebhookError> {
        match &self.authentication_method {
            WebhookAuthenticationMethod::PredefinedProviders(providers) => {
                let handler = providers.webhook_type.get_webhook_handler();

                let challenge_response = handler
                    .0
                    .map(|handler| {
                        handler.handle_challenge_request(headers, &providers, raw_payload)
                    })
                    .transpose()?
                    .flatten();

                if let Some(challenge_response) = challenge_response {
                    return Ok(WebhookRequestType::Challenge(challenge_response));
                }
            }
            WebhookAuthenticationMethod::HMAC(HmacAuthentication {
                webhook_signing_secret,
                config,
            }) => {
                config.validate_hmac_signature(headers, webhook_signing_secret, raw_payload)?;
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
