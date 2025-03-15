use http::HeaderMap;

use super::{HmacAuthenticationData, TryGetWebhookHeader, WebhookAuthenticationData, WebhookError};

pub struct Slack;

impl WebhookAuthenticationData for Slack {
    fn get_authentication_data<'header>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacAuthenticationData<'header>, WebhookError> {
        let slack_secret_signature = headers.try_get_webhook_header("X-Slack-Signature")?;
        let slack_timestamp_header = headers.try_get_webhook_header("X-Slack-Request-Timestamp")?;
        let signed_payload = format!("v0:{}:{}", slack_timestamp_header, raw_payload);

        Ok(HmacAuthenticationData::new(
            signed_payload,
            slack_secret_signature,
            Some("v0="),
        ))
    }
}
