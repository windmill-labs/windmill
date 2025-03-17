use super::{
    Encoding, HmacAlgorithm, HmacAuthenticationData, HmacAuthenticationDetails,
    TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
};
use axum::response::Response;
use http::HeaderMap;
use std::borrow::Cow;

pub struct Slack;

impl WebhookHandler for Slack {
    fn handle_challenge_request<'header>(
        &self,
        _: &'header HeaderMap,
        _: &WebhookAuthenticationMethod,
        _: &str,
    ) -> Result<Option<Response>, WebhookError> {
        Ok(None)
    }

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<HmacAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
        let slack_secret_signature = headers.try_get_webhook_header("X-Slack-Signature")?;
        let slack_timestamp_header = headers.try_get_webhook_header("X-Slack-Request-Timestamp")?;
        let signed_payload = format!("v0:{}:{}", slack_timestamp_header, raw_payload);

        Ok(HmacAuthenticationData::new(
            Cow::Owned(signed_payload),
            slack_secret_signature,
            Some("v0="),
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
        ))
    }
}
