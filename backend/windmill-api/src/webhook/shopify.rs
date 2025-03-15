use http::HeaderMap;

use super::{HmacAuthenticationData, TryGetWebhookHeader, WebhookAuthenticationData, WebhookError};

pub struct Shopify;

impl WebhookAuthenticationData for Shopify {
    fn get_authentication_data<'header>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacAuthenticationData<'header>, WebhookError> {
        let shopify_secret_header = headers.try_get_webhook_header("X-Shopify-Hmac-Sha256")?;
        Ok(HmacAuthenticationData::new(
            raw_payload.to_owned(),
            shopify_secret_header,
            None,
        ))
    }
}
