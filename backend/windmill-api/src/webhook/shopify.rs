use super::{
    Encoding, HmacAlgorithm, HmacAuthenticationData, HmacAuthenticationDetails,
    TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
};
use axum::response::Response;
use http::HeaderMap;
use std::borrow::Cow;
pub struct Shopify;

impl WebhookHandler for Shopify {
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
        let shopify_secret_header = headers.try_get_webhook_header("X-Shopify-Hmac-Sha256")?;
        Ok(HmacAuthenticationData::new(
            Cow::Borrowed(raw_payload),
            shopify_secret_header,
            None,
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Base64),
        ))
    }
}
