use crate::webhook::{PayloadConstruction, SignatureLocation, SignatureParse};

use super::{
    Encoding, HmacAlgorithm, TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError,
    WebhookHandler, WebhookHmacValidator, WebhookVerifier,
};
use axum::{body::Body, response::Response};
use http::HeaderMap;
use serde::Deserialize;
use serde_json::value::RawValue;

lazy_static::lazy_static! {
    pub static ref TWITCH_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
        prefix: Some("sha256=".to_string()),
        payload_construction: PayloadConstruction {
            signature_location: SignatureLocation::Header(SignatureParse {
                signature_header_name: "Twitch-Eventsub-Message-Signature".to_string(),
                parsing_rules: None,
            }),
            payload_format: vec!["Twitch-Eventsub-Message-Id".to_string(), "Twitch-Eventsub-Message-Timestamp".to_string()],
            payload_separator: None,
            include_raw_body_at_end_of_payload: true,
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
        authentication_method: &WebhookAuthenticationMethod,
        raw_payload: &str,
    ) -> Result<Option<Response>, WebhookError> {
        match &authentication_method {
            WebhookAuthenticationMethod::HMAC(hmac) => {
                TWITCH_WEBHOOK_VALIDATOR.validate_hmac_signature(
                    headers,
                    &hmac.webhook_signing_secret,
                    raw_payload,
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

        let twitch_crc_body = serde_json::from_str::<TwitchCrcBody>(raw_payload).map_err(|e| {
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
