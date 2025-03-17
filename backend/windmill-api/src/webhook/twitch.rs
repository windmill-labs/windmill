use super::{
    verify_hmac_signature, Encoding, HmacAlgorithm, HmacAuthenticationData,
    HmacAuthenticationDetails, TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError,
    WebhookHandler,
};
use axum::{body::Body, response::Response};
use http::HeaderMap;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::borrow::Cow;


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
                let authentication_data =
                    self.get_hmac_authentication_data(headers, raw_payload)?;
                verify_hmac_signature(authentication_data, &hmac.webhook_signing_secret)?;
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

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<HmacAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
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

        Ok(HmacAuthenticationData::new(
            Cow::Owned(message),
            twitch_secret_signature,
            Some("sha256="),
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
        ))
    }
}
