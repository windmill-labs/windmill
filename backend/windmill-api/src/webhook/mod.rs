use std::collections::HashMap;

use base64::{
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE},
    Engine,
};
use hmac::{Hmac, Mac};
use http::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;
type HmacSha1 = Hmac<Sha1>;

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
lazy_static::lazy_static! {
    static ref WEBHOOK_TYPE_TO_AUTHENTICATION_METHOD: HashMap<WebhookType, (Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails)> = {
        let mut map = HashMap::new();

        map.insert(WebhookType::Github, (Box::new(Github) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex)));
        map.insert(WebhookType::Shopify, (Box::new(Shopify) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Base64)));
        map.insert(WebhookType::Slack, (Box::new(Slack) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex)));
        map.insert(WebhookType::Stripe, (Box::new(Stripe) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex)));
        map.insert(WebhookType::TikTok, (Box::new(TikTok) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex)));
        map.insert(WebhookType::Twitch, (Box::new(Twitch) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex)));
        map.insert(WebhookType::Zoom, (Box::new(Zoom) as Box<dyn WebhookAuthenticationData + Send + Sync>, HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex)));

        map
    };
}

struct HmacAuthenticationDetails {
    algorithm_to_use: HmacAlgorithm,
    header_key_encoding: Encoding,
}

impl HmacAuthenticationDetails {
    fn new(algorithm_to_use: HmacAlgorithm, header_key_encoding: Encoding) -> Self {
        Self { algorithm_to_use, header_key_encoding }
    }
}

struct HmacAuthenticationData<'header> {
    signed_payload: String,
    header_key_value: &'header str,
    signature_prefix: Option<&'static str>,
}

impl<'header> HmacAuthenticationData<'header> {
    pub fn new(
        signed_payload: String,
        header_key_value: &'header str,
        signature_prefix: Option<&'static str>,
    ) -> Self {
        Self { signed_payload, header_key_value, signature_prefix }
    }
}

trait WebhookAuthenticationData {
    fn get_authentication_data<'header>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacAuthenticationData<'header>, WebhookError>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum HmacAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Serialize, Deserialize)]
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
    webhook_signing_secret: String,
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

trait TryGetWebhookHeader {
    fn try_get_webhook_header<'header>(
        &'header self,
        header_name: &'static str,
    ) -> Result<&'header str, WebhookError>;
}

impl TryGetWebhookHeader for HeaderMap<HeaderValue> {
    fn try_get_webhook_header<'header>(
        &'header self,
        header_name: &'static str,
    ) -> Result<&'header str, WebhookError> {
        let Some(signature_header) = self.get(header_name) else {
            return Err(WebhookError::MissingHeader(header_name));
        };
        let Some(signature_header) = signature_header.to_str().ok() else {
            return Err(WebhookError::InvalidHeader(header_name));
        };

        Ok(signature_header)
    }
}

fn calculate_hmac_signature(algorithm: HmacAlgorithm, secret: &str, payload: &str) -> Vec<u8> {
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
    ) -> Result<(), WebhookError> {
        match &self.authentication_method {
            WebhookAuthenticationMethod::HMAC(HmacAuthentication {
                webhook_signing_secret,
                config,
            }) => {
                match (self.r#type, config.as_ref()) {
                    (WebhookType::Custom, Some(_)) => {}
                    (WebhookType::Custom, None) => {}
                    (_, Some(_)) => {}
                    (_, None) => {}
                }

                let Some((handler, authentication_details)) =
                    WEBHOOK_TYPE_TO_AUTHENTICATION_METHOD.get(&self.r#type)
                else {
                    todo!()
                };

                let authentication_data = handler.get_authentication_data(headers, raw_payload)?;

                let hmac_signature = calculate_hmac_signature(
                    authentication_details.algorithm_to_use,
                    &webhook_signing_secret,
                    &authentication_data.signed_payload,
                );

                let encoded_signature = match authentication_details.header_key_encoding {
                    Encoding::Hex => hex::encode(hmac_signature),
                    Encoding::Base64 => BASE64_STANDARD.encode(hmac_signature),
                    Encoding::Base64Uri => BASE64_URL_SAFE.encode(hmac_signature),
                };

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
                    println!(
                        "HMAC authentication method: Webhook {:?} is invalid",
                        self.r#type
                    );
                    return Err(WebhookError::InvalidSignature);
                } else {
                    println!(
                        "HMAC authentication method: Webhook {:?} is valid",
                        self.r#type
                    );
                }
            }
            WebhookAuthenticationMethod::ApiKey(ApiKeyAuthentication {
                api_key_header,
                api_key,
            }) => {
                todo!()
            }
            WebhookAuthenticationMethod::BasicAuth(BasicAuthAuthentication {
                username,
                password,
            }) => {
                todo!()
            }
        }

        Ok(())
    }

    fn handle_hmac_authentication_method(&self) {}
}

#[derive(thiserror::Error, Debug)]
pub enum WebhookError {
    #[error("failed to parse timestamp")]
    InvalidTimestamp,

    #[error("invalid secret")]
    InvalidSecret(#[from] base64::DecodeError),

    #[error("invalid header {0}")]
    InvalidHeader(&'static str),

    #[error("signature timestamp too old")]
    TimestampTooOldError,

    #[error("signature timestamp too far in future")]
    FutureTimestampError,

    #[error("missing header {0}")]
    MissingHeader(&'static str),

    #[error("signature invalid")]
    InvalidSignature,

    #[error("payload invalid")]
    InvalidPayload,
}
