use hmac::{Hmac, Mac};
use http::HeaderMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
pub enum HmacAlgorithm {
    Sha1,
    Sha256,
    Sha512,
    Md5,
}

#[derive(Deserialize)]
pub enum Encoding {
    Base64,
    Base64Uri,
    Hex,
}

#[derive(Deserialize)]
pub struct HmacAuthentication {
    pub algorithm: HmacAlgorithm,
    pub encoding: Encoding,
    pub signature_header_key: String,
    pub webhook_signing_secret: String,
}

impl HmacAuthentication {
    pub fn new(
        algorithm: HmacAlgorithm,
        encoding: Encoding,
        signature_header_key: String,
        webhook_signing_secret: String,
    ) -> HmacAuthentication {
        HmacAuthentication { algorithm, encoding, signature_header_key, webhook_signing_secret }
    }
}

impl Default for HmacAuthentication {
    fn default() -> Self {
        Self {
            algorithm: HmacAlgorithm::Sha256,
            encoding: Encoding::Base64,
            signature_header_key: "".to_string(),
            webhook_signing_secret: "".to_string(),
        }
    }
}

#[derive(Deserialize)]
pub struct BasicAuthAuthentication {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ApiKeyAuthentication {
    api_key_header: String,
    api_key: String,
}

#[derive(Deserialize)]
pub enum AuthenticationMethod {
    HMAC(HmacAuthentication),
    BasicAuth(BasicAuthAuthentication),
    ApiKey(ApiKeyAuthentication),
}

fn from_provider_to_hmac(provider: WebhookProvider, secret: &str) -> HmacAuthentication {
    match provider {
        WebhookProvider::Stripe => HmacAuthentication { ..Default::default() },
        _ => Default::default(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WebhookProvider {
    AwsSNS,
    Discord,
    Ebay,
    Facebook,
    Github,
    GitLab,
    LinkedIn,
    Linear,
    MailChimp,
    Mailgun,
    Persona,
    Paypal,
    Shopify,
    Slack,
    Stripe,
    Twillio,
    Trello,
    TikTok,
    Twitch,
    Webhook,
    WhatsApp,
    X,
    Zoom,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Webhook {
    pub webhook_provider: WebhookProvider,
    pub signing_secret: String,
}

impl Webhook {
    pub fn verify_signatures(
        &self,
        headers: &HeaderMap,
        raw_payload: &str,
    ) -> Result<bool, WebhookError> {
        let verified = match self.webhook_provider {
            WebhookProvider::Stripe => {
                let Some(stripe_signature) = headers.get("STRIPE-SIGNATURE") else {
                    return Err(WebhookError::MissingHeader("STRIPE-SIGNATURE"));
                };
                let Some(stripe_signature) = stripe_signature.to_str().ok() else {
                    return Err(WebhookError::InvalidHeader("STRIPE-SIGNATURE"));
                };

                let mut splitted_signature = stripe_signature
                    .split(',')
                    .map(|value| {
                        let mut s = value.split('=');
                        (s.next().unwrap(), s.next().unwrap())
                    })
                    .collect_vec();

                println!("{:#?}", &splitted_signature);

                if splitted_signature.len() < 1 {
                    return Err(WebhookError::InvalidHeader("STRIPE-SIGNATURE"));
                }

                let timestamp = match splitted_signature.get(0).unwrap() {
                    ("t", timestamp) => timestamp,
                    val => {
                        println!("Error in timestamp: {:#?}", val);
                        return Err(WebhookError::InvalidHeader("STRIPE-SIGNATURE"));
                    }
                };

                let mut hmac = HmacSha256::new_from_slice(self.signing_secret.as_bytes().into())
                    .expect("Size");
                let signed_payload = format!("{}.{}", timestamp, raw_payload);
                println!("Signed payload: {}", &signed_payload);
                hmac.update(signed_payload.as_bytes());

                let verified_message = hmac.finalize().into_bytes();
                for (version, signature) in splitted_signature.iter().skip(1) {
                    if *version == "v1" {
                        let signature = hex::decode(*signature)
                            .map_err(|_| WebhookError::InvalidHeader("STRIPE-SIGNATURE"))?;
                        println!("{:#?} {:#?}", &verified_message, &signature);
                        if verified_message.as_slice() == signature.as_slice() {
                            println!("Matched!")
                        } else {
                            println!("Not Matched!")
                        }
                    }
                }

                let timestamp = timestamp
                    .parse::<i64>()
                    .map_err(|_| WebhookError::InvalidHeader("STRIPE-SIGNATURE"))?;

                true
            }
            _ => false,
        };

        Ok(verified)
    }
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
