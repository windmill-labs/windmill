use http::HeaderMap;

use super::{HmacAuthenticationData, TryGetWebhookHeader, WebhookAuthenticationData, WebhookError};

pub struct Twitch;

impl WebhookAuthenticationData for Twitch {
    fn get_authentication_data<'header>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacAuthenticationData<'header>, WebhookError> {
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
            message,
            twitch_secret_signature,
            Some("sha256="),
        ))
    }
}
