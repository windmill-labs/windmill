use http::HeaderMap;

use super::{HmacAuthenticationData, TryGetWebhookHeader, WebhookAuthenticationData, WebhookError};

pub struct Zoom;

impl WebhookAuthenticationData for Zoom {
    fn get_authentication_data<'header>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacAuthenticationData<'header>, WebhookError> {
        let zoom_signature_header = headers.try_get_webhook_header("x-zm-signature")?;
        let zoom_timestamp_header = headers.try_get_webhook_header("x-zm-request-timestamp")?;

        let message = format!("v0:{}:{}", zoom_timestamp_header, raw_payload);

        Ok(HmacAuthenticationData::new(
            message,
            zoom_signature_header,
            Some("v0="),
        ))
    }
}
